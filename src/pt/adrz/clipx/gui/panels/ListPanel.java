package pt.adrz.clipx.gui.panels;

import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;

import javax.swing.JList;
import javax.swing.JMenuItem;
import javax.swing.JPanel;
import javax.swing.JPopupMenu;
import javax.swing.SwingUtilities;

public abstract class ListPanel extends JPanel implements MouseListener {
	private static final long serialVersionUID = 1L;

	public enum MenuLabels { activate,edit,delete }
	
	protected JPopupMenu rightClickMenu;

	public abstract void activate();
	public abstract void edit();
	public abstract void delete();
	public abstract void showRightClickMenu(MouseEvent e);
	
	public ListPanel() {
		createRightClickMenu();
	}
	
	private void actionMenuClick(MouseEvent e) {

		JMenuItem item = (JMenuItem)e.getSource();
		MenuLabels label = MenuLabels.valueOf(item.getText());
		
		switch (label) {
			case activate:
				activate();
				break;
			case edit:
				edit();
				break;
			case delete:
				delete();
				break;
			default:
				break;
		}
	}
	
	private void createRightClickMenu() {

		this.rightClickMenu = new JPopupMenu();

		JMenuItem jmActivate = new JMenuItem(MenuLabels.activate.toString());
		JMenuItem jmEdit = new JMenuItem(MenuLabels.edit.toString());
		JMenuItem jmDelete = new JMenuItem(MenuLabels.delete.toString());

		this.rightClickMenu.add(jmActivate);
		this.rightClickMenu.add(jmEdit);
		this.rightClickMenu.add(jmDelete);

		jmActivate.addMouseListener(this);
		jmEdit.addMouseListener(this);
		jmDelete.addMouseListener(this);
	}
	
	@Override
	public void mousePressed(MouseEvent e) {
		
		if ( e.getSource() instanceof JList<?> && SwingUtilities.isRightMouseButton(e) ) {
			showRightClickMenu(e);
			return;
		}

		if ( e.getSource() instanceof JMenuItem ) {
			this.actionMenuClick(e);
			return;
		}
	}
	
	@Override
	public void mouseClicked(MouseEvent e) {

	}

	@Override
	public void mouseReleased(MouseEvent e) {
		
	}

	@Override
	public void mouseEntered(MouseEvent e) {
		
	}

	@Override
	public void mouseExited(MouseEvent e) {
		
	}
}
