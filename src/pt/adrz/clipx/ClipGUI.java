/**
 * ClipGui
 */

package pt.adrz.clipx;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Container;
import java.awt.Menu;
import java.awt.event.KeyEvent;
import java.awt.event.KeyListener;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;

import javax.swing.JFrame;
import javax.swing.JMenuBar;
import javax.swing.JMenuItem;
import javax.swing.JPanel;
import javax.swing.JScrollPane;
import javax.swing.JTextArea;
import javax.swing.ListSelectionModel;
import javax.swing.ScrollPaneConstants;
import javax.swing.event.DocumentEvent;
import javax.swing.event.DocumentListener;
import javax.swing.event.ListSelectionEvent;
import javax.swing.event.ListSelectionListener;

public class ClipGUI extends JFrame implements ListSelectionListener, DocumentListener, KeyListener {

	private Container 			container;
	
	private int 				xWindowDim = 600;
	private int 				yWindowDim = 400;
	private int 				visibleListRowCount = 10;
	
	private JPanel				panel1;
	private JPanel				panel2;
	final private JTextArea		editTA;
	private JScrollPane			textAreaScrollPane;
	
	// menus
	private JMenuBar			menuBar;
	private Menu				menu1;
	private JMenuItem			menu1Item1, menu1Item2, menu1Item3;
	
	// List
	private ClipList			list;
	private JScrollPane			listScrollPane;
	
	// reference to ClipManager
	ClipManager clipManager;
	
	
	
	/**
	 * Constructor
	 */
	public ClipGUI(final ClipManager clipManager) {
		super("ClipX");
		this.clipManager = clipManager;
		container = this.getContentPane();
		container.setLayout(new BorderLayout());
		
		// List
		list = new ClipList();
		listScrollPane = new JScrollPane();
		list.setSelectionMode(ListSelectionModel.SINGLE_SELECTION);
		list.setSelectedIndex(0);
		list.addListSelectionListener(this);
		list.setVisibleRowCount(visibleListRowCount);
		listScrollPane = new JScrollPane(list,ScrollPaneConstants.VERTICAL_SCROLLBAR_ALWAYS,ScrollPaneConstants.HORIZONTAL_SCROLLBAR_NEVER);
		list.setPrototypeCellValue("tamanho"); // set horizontal size
		
		// Detect double click events in the list
		MouseListener mouseListener = new MouseAdapter() {
			public void mouseClicked(MouseEvent e) {
				if (e.getClickCount() == 2) {
					
					// get the selected string from the filteredlist
					String selectedString = (String)list.getModel().getElementAt(list.locationToIndex(e.getPoint()));
					
					// get the position from all the the items
					int pos = list.getModel().getItems().indexOf(selectedString);
					
					// set the clipboard
					clipManager.setClipboard(selectedString);
					
					list.getModel().switchVals(pos, selectedString);
					
					list.setSelectedIndex(0);
					list.getFilterField().setText("");
					getEditTA().setText(selectedString);
				}
			}
		};
		
		// add listeners
		list.addMouseListener(mouseListener);
		list.addKeyListener(this);
		
		// create panels
		panel1 = new JPanel();
		panel2 = new JPanel();
		panel1.setLayout(new BorderLayout());
		panel2.setLayout(new BorderLayout());	
		panel1.setBackground(Color.green);
		panel2.setBackground(Color.blue);
		
		// TextArea
		editTA 				= new JTextArea();
		getEditTA().getDocument().addDocumentListener(this);
		textAreaScrollPane 	= new JScrollPane(getEditTA(),ScrollPaneConstants.VERTICAL_SCROLLBAR_ALWAYS,ScrollPaneConstants.HORIZONTAL_SCROLLBAR_ALWAYS);
		
		// add items
		container.add(panel1, BorderLayout.WEST);
		container.add(panel2, BorderLayout.CENTER);	
		panel1.add(list.getFilterField(), BorderLayout.NORTH);
		panel1.add(listScrollPane, BorderLayout.CENTER);
		panel2.add(textAreaScrollPane, BorderLayout.CENTER);
		
		this.setSize(xWindowDim, yWindowDim);
		this.setLocationRelativeTo(null);
		this.setDefaultCloseOperation(HIDE_ON_CLOSE);
		this.setVisible(true);
	}

	
	

	
	public ClipList getList() {
		return this.list;
	}
	

	/**
	 * Get Text Area
	 * @return the editTA
	 */
	public JTextArea getEditTA() {
		return editTA;
	}





	/**
	 * Detects changes in the list
	 */
	@Override
	public void valueChanged(ListSelectionEvent e) {	
		// whenever the user makes a selection in the list, the text will be placed in the text area
		if (e.getValueIsAdjusting()) {
			return;
		}	
		else {
			getEditTA().setText((String)list.getModel().getElementAt(list.getSelectedIndex()));
		}
	}





	@Override
	public void changedUpdate(DocumentEvent e) {
		// TODO Auto-generated method stub
		String tmp = getEditTA().getText();
		System.out.println("selected index = " + list.getSelectedIndex());
	}





	@Override
	public void insertUpdate(DocumentEvent e) {
		// TODO Auto-generated method stub
		System.out.println("selected index = " + list.getSelectedIndex());
	}





	@Override
	public void removeUpdate(DocumentEvent e) {
		// TODO Auto-generated method stub
		System.out.println("selected index = " + list.getSelectedIndex());
	}



	
	
	
	
	
	
	
	// keylistener **************

	
	/**
	 * Event when some key is pressed. This listener is only added to the jlist component
	 * So far, only the delete key is implemented
	 */
	@Override
	public void keyPressed(KeyEvent arg0) {
		if (arg0.getKeyCode() == KeyEvent.VK_DELETE) {
			try {
				int index = list.getSelectedIndex();
				list.getModel().remove(index);
				if (index == 0) {
				
				}
			}
			catch (IndexOutOfBoundsException eIndexOutBound) {
				
			}
		}
			
	}

	@Override
	public void keyReleased(KeyEvent arg0) {
		// TODO Auto-generated method stub
		
	}

	@Override
	public void keyTyped(KeyEvent arg0) {
		// TODO Auto-generated method stub
		
	}

}
