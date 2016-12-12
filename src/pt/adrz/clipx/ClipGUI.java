/**
 * ClipGui
 */

package pt.adrz.clipx;

import java.awt.BorderLayout;
import java.awt.Container;
import java.awt.FlowLayout;

import javax.swing.JFrame;
import javax.swing.JPanel;
import javax.swing.JScrollPane;
import javax.swing.JTextArea;
import javax.swing.ScrollPaneConstants;

import pt.adrz.clipx.gui.panels.CentralPanel;
import pt.adrz.clipx.gui.panels.LeftPanel;
import pt.adrz.clipx.gui.panels.Panels;
import pt.adrz.clipx.gui.panels.RightPanel;

public class ClipGUI extends JFrame implements Panels {
	private static final long serialVersionUID = 4285795541593969626L;
	private static final String TITLE = "ClipX";

	private int 				xWindowDim = 1000;
	private int 				yWindowDim = 400;

	private Container 			container;
	
	private LeftPanel				leftPanel;
	private JPanel				centralPanel;
	private RightPanel			rightPanel;

	private JTextArea		 	editTA;
	private JScrollPane			textAreaScrollPane;
	
	private ClipSysTray 		clipSysTray;
	private ClipMenuBar			clipMenuBar;
	
	private ClipManager 		clipManager;
	
	public ClipGUI() {
		
		super(TITLE);
		
		this.clipManager = new ClipManager();

		this.clipSysTray = new ClipSysTray(this);
		this.clipSysTray.setEnableListener(clipManager);
		
		this.clipMenuBar = new ClipMenuBar();
		this.clipMenuBar.setEnableListener(clipManager);
		
		this.setJMenuBar(this.clipMenuBar);
		
		this.createGUI();
	}
	
	private void tmp() {

	}

	private void createGUI() {
		
		Container centerContainer = new Container();
		centerContainer.setLayout(new FlowLayout());
		container = this.getContentPane();
		container.setLayout(new BorderLayout());
		
		leftPanel = new LeftPanel();
		centralPanel = new JPanel();
		rightPanel = new RightPanel();

		leftPanel.setPanels(this);
		rightPanel.setPanels(this);

		clipManager.addClipboardListener(leftPanel);
		clipManager.setPanels(this);

		editTA = new JTextArea();
		editTA.setEditable(false);
		textAreaScrollPane 	= new JScrollPane(this.editTA,ScrollPaneConstants.VERTICAL_SCROLLBAR_ALWAYS,ScrollPaneConstants.HORIZONTAL_SCROLLBAR_ALWAYS);

		centralPanel.setLayout(new BorderLayout());	
		centralPanel.add(textAreaScrollPane, BorderLayout.CENTER);

		//centerContainer.add(leftPanel);
		//centerContainer.add(rightPanel);
		//container.add(centerContainer, BorderLayout.NORTH);	
		//container.add(centralPanel,BorderLayout.CENTER);	
		
		container.add(leftPanel, BorderLayout.CENTER);
		container.add(rightPanel, BorderLayout.EAST);	
		container.add(centralPanel, BorderLayout.SOUTH);
		//container.add(rightPanel,BorderLayout.EAST);
		
		this.setSize(xWindowDim, yWindowDim);
		//this.setMinimumSize(new Dimension(xWindowDim, yWindowDim));
		this.setLocationRelativeTo(null);
		this.setDefaultCloseOperation(HIDE_ON_CLOSE);
		this.setVisible(true);
	}

	@Override
	public void changeClipBoard(String text) {
		clipManager.setClipboard(text);
	}

	@Override
	public JTextArea getTextArea() {
		return editTA;
	}

	@Override
	public LeftPanel getLeftPanel() {
		return leftPanel;
	}

	@Override
	public RightPanel getRightPanel() {
		return rightPanel;
	}

	@Override
	public CentralPanel getCentralPanel() {
		return null;
	}
}