package pt.adrz.clipx.gui.panels;

import java.awt.BorderLayout;
import java.awt.FlowLayout;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;
import java.util.Arrays;
import java.util.LinkedList;

import javax.swing.JButton;
import javax.swing.JPanel;
import javax.swing.JScrollPane;
import javax.swing.JTextField;
import javax.swing.ListSelectionModel;
import javax.swing.ScrollPaneConstants;

import pt.adrz.clipx.gui.list.Node;
import pt.adrz.clipx.gui.list.RightList;

public class RightPanel extends ListPanel implements ActionListener, MouseListener {
	private static final long serialVersionUID = 1L;
	
	private RightList list;
	private JScrollPane scrollPane;
	private JPanel southPanel;
	private JTextField inputNewElement;
	private JButton addButton;

	private Panels panels;
	
	public RightPanel() {
		this.createList();
		this.createSouthElements();
		this.addComponents();
	}
	
	private void createList() {
		list = new RightList();

		list.getModel().addElement(new Node("first", new LinkedList<String>(Arrays.asList("ceasd","asdf"))));
		list.getModel().addElement(new Node("secount", new LinkedList<String>(Arrays.asList("2134","4312"))));
		list.getModel().addElement(new Node("thirth", new LinkedList<String>(Arrays.asList("c85757","a5757"))));

		list.setSelectedIndex(0);
		list.setVisibleRowCount(10);
		list.setSelectionMode(ListSelectionModel.SINGLE_SELECTION);

		list.addMouseListener(this);

		scrollPane = new JScrollPane();
		scrollPane = new JScrollPane(list,ScrollPaneConstants.VERTICAL_SCROLLBAR_ALWAYS,ScrollPaneConstants.HORIZONTAL_SCROLLBAR_NEVER);
	}
	
	private void createSouthElements() {
		inputNewElement = new JTextField(20);
		addButton = new JButton("NEW");

		southPanel = new JPanel();
		southPanel.setLayout(new FlowLayout());
		southPanel.add(inputNewElement);
		southPanel.add(addButton);
		
		addButton.addActionListener(this);
	}
	
	private void addComponents() {
		this.setLayout(new BorderLayout());
		this.add(list.getSearchField(),BorderLayout.NORTH);
		this.add(scrollPane,BorderLayout.CENTER);
		this.add(southPanel, BorderLayout.SOUTH);
	}

	public void setPanels(Panels panels) {
		this.panels = panels;
	}
	
	public Panels getPanels() {
		return this.panels;
	}
	
	@Override
	public void mouseClicked(MouseEvent e) {
		if ( e.getClickCount() == 2 ) {
			
			int index = list.locationToIndex(e.getPoint());
			
			// double click in an empty space
			if ( index == -1 ) {
				list.clearSelection();
				return;
			}
			
			Node node = this.list.getModel().getElementAt(index);
			
			list.getModel().switchVals(list.getModel().getItems().indexOf(node), node);
			list.setSelectedIndex(0);
			list.getSearchField().setText("");

			this.panels.getLeftPanel().getList().getModel().setItems(node.getItems());
		}
	}

	@Override
	public void actionPerformed(ActionEvent e) {
		if ( !inputNewElement.getText().isEmpty() )
			list.getModel().addElement(new Node(inputNewElement.getText()));
	}

	@Override
	public void activate() {
		System.out.println("activate");
	}

	@Override
	public void edit() {
		System.out.println("edit");
	}

	@Override
	public void delete() {
		System.out.println("delete");
	}

	@Override
	public void showRightClickMenu(MouseEvent e) {
		list.setSelectedIndex(list.locationToIndex(e.getPoint()));
		rightClickMenu.show(e.getComponent(), e.getX(), e.getY());
	}
}
